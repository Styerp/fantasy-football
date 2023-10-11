use clap::Parser;
use espn_fantasy_football::api::{
    client::EspnClient,
    id_maps::PositionId,
    matchup::{Matchup, RosterSlot, TeamMatchupPerformance},
    player::PlayerId,
    team::TeamId,
};
use std::{collections::HashMap, ops::Add};
#[derive(Parser)]
struct Args {
    #[arg(long, env = "SWID", long_help = "SWID Cookie value from espn.com")]
    swid: Option<String>,
    #[arg(
        long,
        env = "ESPN_S2",
        long_help = "ESPN_S2 Cookie value from espn.com"
    )]
    espn_s2: Option<String>,
    #[arg(
        short,
        long,
        env = "ESPN_LEAGUE_ID",
        long_help = "ESPN Fantasy League Identifier"
    )]
    league: i32,
    #[arg(short, long, long_help = "The year of the season.")]
    season: i16,
    #[arg(short, long, long_help = "The week of the season.")]
    week: u8,
    #[arg(
        short,
        long,
        long_help = r#"When true, will sum all weeks until the specified week and report on bench king across all weeks in aggregate."#
    )]
    comprehensive: bool,
}

#[tokio::main]
async fn main() {
    let cli = Args::parse();
    let client = EspnClient::build(
        &cli.swid.clone().unwrap(),
        &cli.espn_s2.unwrap(),
        cli.league,
    );
    let league_settings = client.get_league_settings(cli.season).await;

    let teams = client.get_team_data(cli.season).await;
    let roster_target = roster_for_slotting(league_settings.roster_settings.lineup_slot_counts);

    if !cli.comprehensive {
        let matchup_data = client
            .get_matchups_for_week(cli.season, cli.week, cli.week)
            .await;
        let deets = bench_king_details_for_week(matchup_data, &roster_target);
        let standings = bench_king_standing_from_team_map(deets);
        let the_king = standings.get(0).unwrap();

        println!(
            "Bench King for Week {} was {} with {} points left benched.",
            cli.week,
            teams.iter().find(|x| &x.id == &the_king.0).unwrap().name,
            the_king.1.suboptimal_points()
        );

        println!("======= Week Standings =======");
        for (index, (team, stats)) in standings.iter().enumerate() {
            let team_detail = teams.iter().find(|x| &x.id == team).unwrap();
            println!(
                "In place {}, team {} left {} points on the bench.",
                index + 1,
                team_detail.name,
                stats.suboptimal_points().round()
            )
        }
    } else {
        let mut team_score_by_week: HashMap<TeamId, HashMap<u8, ProgressTracker>> = HashMap::new();
        for week in 1..=cli.week {
            let matchup_data = client.get_matchups_for_week(cli.season, week, week).await;
            let deets = bench_king_details_for_week(matchup_data, &roster_target);
            for (team, data) in deets {
                team_score_by_week
                    .entry(team)
                    .or_insert(HashMap::new())
                    .entry(week)
                    .or_insert(data);
            }
        }

        let final_tally =
            team_score_by_week
                .iter()
                .fold(HashMap::new(), |mut acc, (team, week_data)| {
                    let total = week_data.iter().fold(
                        ProgressTracker {
                            actual_points: 0.0,
                            optimal_points: 0.0,
                            zero_point_starters: 0,
                        },
                        |gather, (_week, data)| gather + *data,
                    );
                    acc.entry(team.clone()).or_insert(total);
                    acc
                });
        let standings = bench_king_standing_from_team_map(final_tally);
        let the_king = standings.get(0).unwrap();
        println!(
            "Bench King through week {} was {} with {} points left benched.",
            cli.week,
            teams.iter().find(|x| &x.id == &the_king.0).unwrap().name,
            the_king.1.suboptimal_points()
        );
        println!("======= Overall Standings =======");
        for (index, (team, stats)) in standings.iter().enumerate() {
            let team_detail = teams.iter().find(|x| &x.id == team).unwrap();
            println!(
                "In place {}, team {} left ~{} points on the bench.",
                index + 1,
                team_detail.name,
                stats.suboptimal_points().round(),
            )
        }
    }
}

fn bench_king_standing_from_team_map(
    data: HashMap<TeamId, ProgressTracker>,
) -> Vec<(TeamId, ProgressTracker)> {
    let mut output = data.iter().map(|x| (*x.0, *x.1)).collect::<Vec<_>>();
    output.sort_by_key(|x| x.1.suboptimal_points() as i32);
    output.reverse();
    output
}

fn bench_king_details_for_week(
    data: Vec<Matchup>,
    roster_target: &Vec<(PositionId, i8)>,
) -> HashMap<TeamId, ProgressTracker> {
    let mut week_data = HashMap::new();

    for matchup in data {
        let p0 = analyze_performance(&matchup.away, &roster_target);
        let p1 = analyze_performance(&matchup.home, &roster_target);
        week_data.entry(matchup.away.team_id.clone()).or_insert(p0);
        week_data.entry(matchup.home.team_id.clone()).or_insert(p1);
    }
    week_data
}

/// Takes a map of Positions and their limits and creates a sorted
fn roster_for_slotting(position_limits: HashMap<PositionId, i8>) -> Vec<(PositionId, i8)> {
    let mut roster = HashMap::new();
    for (position, count) in position_limits {
        if !["Bench", "IR"].contains(&position.to_string()) && count > 0 {
            roster.entry(position.clone()).or_insert(count.clone());
        }
    }
    let mut roster = roster
        .iter()
        .map(|(x, y)| (x.to_owned(), y.to_owned()))
        .collect::<Vec<_>>();
    roster.sort_by_key(|(x, _y)| x.to_string().len());
    roster
}

fn analyze_performance(
    performance: &TeamMatchupPerformance,
    slots: &Vec<(PositionId, i8)>,
) -> ProgressTracker {
    let mut roster = match &performance.roster_for_current_scoring_period {
        Some(r) => r.entries.clone(),
        None => panic!("No roster"),
    };
    roster.sort_by_key(|x| x.player_pool_entry.applied_stat_total as i64);
    roster.reverse();

    let optimal = slot_in_roster(roster, slots, performance.total_points);
    return optimal;
}

fn slot_in_roster(
    people: Vec<RosterSlot>,
    roster: &Vec<(PositionId, i8)>,
    actual: f32,
) -> ProgressTracker {
    let mut opt = ProgressTracker {
        optimal_points: 0.0,
        actual_points: actual,
        zero_point_starters: people.iter().fold(0, |mut acc, x| {
            if [PositionId(21), PositionId(23)].contains(&x.lineup_slot_id)
                && x.player_pool_entry.applied_stat_total == 0.0
            {
                acc += 1;
            };
            acc
        }),
    };
    let mut drafted: Vec<PlayerId> = Vec::new();
    for slot in roster {
        for _i in 1..=slot.1 {
            for person in &people {
                if drafted.contains(&person.player_id) {
                    continue;
                } else if person
                    .player_pool_entry
                    .player
                    .eligible_slots
                    .contains(&slot.0)
                {
                    opt.optimal_points += person.player_pool_entry.applied_stat_total;
                    drafted.push(person.player_id);
                    break;
                }
            }
        }
    }
    return opt;
}
#[derive(Clone, Copy, Debug)]
pub struct ProgressTracker {
    pub optimal_points: f32,
    pub actual_points: f32,
    pub zero_point_starters: i8,
}
impl ProgressTracker {
    pub fn suboptimal_points(&self) -> f32 {
        self.optimal_points - self.actual_points
    }
}
impl Add for ProgressTracker {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            zero_point_starters: self.zero_point_starters + other.zero_point_starters,
            optimal_points: self.optimal_points + other.optimal_points,
            actual_points: self.actual_points + other.actual_points,
        }
    }
}

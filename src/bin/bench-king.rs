use std::collections::HashMap;
use clap::Parser;
use espn_fantasy_football::api::{
    client::EspnClient,
    id_maps::PositionId,
    matchup::{Matchup, RosterSlot, TeamMatchupPerformance},
    player::PlayerId,
    team::TeamId,
};
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
    #[arg(short, long, long_help = "The week of the season")]
    week: u8,
    #[arg(
        short,
        long,
        long_help = "When true, will operate on all weeks of the season to the specified week."
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
        let the_king = deets
            .iter()
            .max_by_key(|x| (x.1.optimal_points - x.1.actual_points) as i32);
        let the_king = the_king.unwrap();
        println!(
            "Bench King for Week {} was {} with {} points left benched.",
            cli.week,
            teams.iter().find(|x| &x.id == the_king.0).unwrap().name,
            the_king.1.suboptimal_points()
        )
    } else {
        //TODO
    
        let mut team_score_by_week: HashMap<TeamId, HashMap<u8, ProgressTracker>> = HashMap::new();
        for week in 1..cli.week {
            let matchup_data = client
                .get_matchups_for_week(cli.season, cli.week, cli.week)
                .await;
            let deets = bench_king_details_for_week(matchup_data, &roster_target);
            //weeks.entry(week).or_insert(deets);
            for (team, data) in deets {
                team_score_by_week.entry(team)
                .or_insert(HashMap::new())
                .entry(week).or_insert(data);
    
            }
        }
        //team_score_by_week.iter().fold(HashMap::new())
    }
    // for (team, data) in deets {
    //     let team_data = teams.iter().find(|x| x.id == team).unwrap();
    //     println!("Team {}; {:?}", team_data.name, data)
    // }
}

fn bench_king_details_for_week(
    data: Vec<Matchup>,
    roster_target: &Vec<(PositionId, i8)>,
) -> HashMap<TeamId, ProgressTracker> {
    let mut week_data = HashMap::new();

    for matchup in data {
        let mut p0 = analyze_performance(&matchup.away, &roster_target);
        let mut p1 = analyze_performance(&matchup.home, &roster_target);
        if p0.actual_points > p1.actual_points {
            p0.won_game = true;
        } else { p1.won_game = true }
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
        won_game: false
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
#[derive(Clone, Debug)]
pub struct ProgressTracker {
    pub optimal_points: f32,
    pub actual_points: f32,
    pub zero_point_starters: i8,
    pub won_game: bool,
}
impl ProgressTracker {
    pub fn suboptimal_points(&self) -> f32
 {
    self.optimal_points - self.actual_points
 }}
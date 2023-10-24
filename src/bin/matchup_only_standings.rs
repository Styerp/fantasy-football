use espn_fantasy_football::client::EspnClient;
use std::collections::HashMap;

use clap::Parser;

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
    season: u16,
    // #[arg(short, long, long_help = "The week of the season")]
    // week: u8
}

pub struct SimpleRecord {
    wins: u8,
    losses: u8,
    ties: u8,
    points_for: f32,
    points_against: f32,
}

#[tokio::main]
async fn main() {
    let cli = Args::parse();
    let client = EspnClient::build(cli.league, &cli.swid.unwrap(), &cli.espn_s2.unwrap());
    let teams = client.teams_for_season(cli.season).await;
    let matchups = client.get_matchups(cli.season).await;
    let mut records = HashMap::new();
    for (team_id, _) in &teams {
        records.entry(team_id.clone()).or_insert(SimpleRecord {
            wins: 0,
            losses: 0,
            ties: 0,
            points_for: 0.0,
            points_against: 0.0,
        });
    }
    for matchup in matchups {
        let winner = matchup.winner;
        let home = matchup.home.unwrap();
        let away = matchup.away.unwrap();
        if winner == "HOME" {
            records.entry(home.team_id).and_modify(|x| {
                x.wins += 1;
                x.points_against += away.total_points;
                x.points_for += home.total_points;
            });
            records.entry(away.team_id).and_modify(|x| {
                x.losses += 1;

                x.points_against += home.total_points;
                x.points_for += away.total_points;
            });
        } else if winner == "AWAY" {
            records.entry(away.team_id).and_modify(|x| {
                x.wins += 1;

                x.points_against += home.total_points;
                x.points_for += away.total_points;
            });

            records.entry(home.team_id).and_modify(|x| {
                x.losses += 1;

                x.points_against += away.total_points;
                x.points_for += home.total_points;
            });
        }
    }
    let mut standings = records
        .into_iter()
        .map(|(x, y)| {
            let (_team_id, team) = teams.iter().find(|t| t.0 == &x).unwrap();
            (team.name.clone(), y)
        })
        .collect::<Vec<_>>();
    standings.sort_by_key(|a| (a.1.wins, a.1.points_for as i32, a.1.points_against as i32));
    standings.reverse();
    // for team in teams.teams {
    //     println!("{:?}", team.record)
    // }
    for (index, (team, record)) in standings.iter().enumerate() {
        println!(
            "In {} place, team {} with a record of {}-{}-{}. Pts For: {}",
            index + 1,
            team,
            record.wins,
            record.losses,
            record.ties,
            record.points_for as i32
        );
    }
}

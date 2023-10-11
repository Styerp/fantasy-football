use espn_fantasy_football::api::client::EspnClient;
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
    season: i16,
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
    let client = EspnClient::build(&cli.swid.unwrap(), &cli.espn_s2.unwrap(), cli.league);
    let teams = client.get_team_data(cli.season).await;
    let matchups = client.get_matchups(cli.season).await;
    let mut records = HashMap::new();
    for team in &teams {
        records.entry(team.id.clone()).or_insert(SimpleRecord {
            wins: 0,
            losses: 0,
            ties: 0,
            points_for: 0.0,
            points_against: 0.0,
        });
    }
    for matchup in matchups.schedule {
        let winner = matchup.winner;
        if winner == "HOME" {
            records.entry(matchup.home.team_id).and_modify(|x| {
                x.wins += 1;
                x.points_against += matchup.away.total_points;
                x.points_for += matchup.home.total_points;
            });
            records.entry(matchup.away.team_id).and_modify(|x| {
                x.losses += 1;

                x.points_against += matchup.home.total_points;
                x.points_for += matchup.away.total_points;
            });
        } else if winner == "AWAY" {
            records.entry(matchup.away.team_id).and_modify(|x| {
                x.wins += 1;

                x.points_against += matchup.home.total_points;
                x.points_for += matchup.away.total_points;
            });

            records.entry(matchup.home.team_id).and_modify(|x| {
                x.losses += 1;

                x.points_against += matchup.away.total_points;
                x.points_for += matchup.home.total_points;
            });
        }
    }
    let mut standings = records
        .into_iter()
        .map(|(x, y)| {
            let team = teams.iter().find(|t| t.id == x).unwrap();
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

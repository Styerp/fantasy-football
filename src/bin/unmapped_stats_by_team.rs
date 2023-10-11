use espn_fantasy_football::api::{client::EspnClient, team::Team};
use std::env;

#[tokio::main]
async fn main() {
    let swid = env::var("SWID").unwrap();
    let espn_s2 = env::var("ESPN_S2").unwrap();
    let league = env::var("ESPN_LEAGUE_ID")
        .unwrap()
        .parse::<i32>()
        .expect("Number");
    let client = EspnClient::build(&swid, &espn_s2, league);
    let teams: Vec<Team> = client.get_team_data(2023).await;
    for team in &teams {
        for (stat, val) in &team.values_by_stat {
            if stat.to_name() == "Unknown" && val > &0f32 {
                println!("Team {} has stat {:?} with value {}", team.name, stat, val)
            }
        }
    }
}

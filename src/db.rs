use tokio_postgres::tls::NoTls;
use tokio_postgres::*;

pub async fn get_connection(login_info: &str) -> Client {
    let (client, connection) = tokio_postgres::connect(login_info, NoTls).await.unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client
}

pub async fn start_sending_to_server(server_id: i64, channel_id: i64, client: &Client) {
    let result = client
        .query("SELECT * FROM servers WHERE server_id = $1", &[&server_id])
        .await
        .unwrap();

    if result.len() == 0 {
        println!("Added");
        client
            .execute(
                "INSERT INTO servers (server_id, channel_id) VALUES ($1, $2)",
                &[&server_id, &channel_id],
            )
            .await
            .unwrap();
    } else {
        println!("Updated");
        client
            .execute(
                "UPDATE servers SET channel_id = $1 where server_id = $2",
                &[&channel_id, &server_id],
            )
            .await
            .unwrap();
    }
}

pub async fn stop_sending_to_server(server_id: i64, client: &Client) {
    client
        .execute("DELETE FROM servers WHERE server_id = $1", &[&server_id])
        .await
        .unwrap();
}

pub async fn get_channel_ids(client: &Client) -> Vec<i64> {
    client
        .query("SELECT channel_id FROM servers", &[])
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect::<Vec<i64>>()
}

pub async fn mark_submission_as_viewed(submission_id: i64, client: &Client) {
    client
        .execute(
            "INSERT INTO sent_protogens (submission_id) VALUES ($1)",
            &[&submission_id],
        )
        .await
        .unwrap();
}

pub async fn has_submission_been_viewed(submission_id: i64, client: &Client) -> bool {
    let results = client
        .query(
            "SELECT * FROM sent_protogens WHERE submission_id = $1",
            &[&submission_id],
        )
        .await
        .unwrap();

    results.len() == 1
}

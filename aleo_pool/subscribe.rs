use goose::prelude::*;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        // In this example, we only create a single scenario, named "WebsiteUser".
        .register_scenario(
            scenario!("Subscribe")
                // After each transactions runs, sleep randomly from 5 to 15 seconds.
                // .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                // This transaction only runs one time when the user first starts.
                // .register_transaction(transaction!(website_login).set_on_start())
                // These next two transactions run repeatedly as long as the load test is running.
                .register_transaction(transaction!(subscribe))
                // .register_transaction(transaction!(authorize))
                // .register_transaction(transaction!(submit))
            // .register_transaction(transaction!(website_about)),
        )
        .execute()
        .await?;

    Ok(())
}

async fn subscribe(user: &mut GooseUser) -> TransactionResult {
    let _goose = user.aleo_sub("/subscribe").await?;

    Ok(())
}



use goose::prelude::*;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        // In this example, we only create a single scenario, named "WebsiteUser".
        .register_scenario(
            scenario!("Authorize")
            .register_transaction(transaction!(authorize))
        )
        .execute()
        .await?;

    Ok(())
}

async fn authorize(user: &mut GooseUser) -> TransactionResult {
    let _goose = user.aleo_authorize("/authorize").await?;

    Ok(())
}
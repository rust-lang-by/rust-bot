[![Build and Test](https://github.com/rust-lang-by/rust-bot/actions/workflows/build.yml/badge.svg)](https://github.com/rust-lang-by/rust-bot/actions/workflows/build.yml) [![Deploy](https://github.com/rust-lang-by/rust-bot/actions/workflows/deploy.yml/badge.svg)](https://github.com/rust-lang-by/rust-bot/actions/workflows/deploy.yml)

# rust-bot
Telegram bot triggered by rust word.


# How to Run

1. Declare the env variables

    ```$ export TELOXIDE_TOKEN=<Your token here> ```

    ```$ export DATABASE_URL=<Your postgress db url here>```


2. Create the database.

    ```$ sqlx db create```


3. Run sql migrations

    ```$ sqlx migrate run```

   
4. Build and run application

   ```$ cargo run```

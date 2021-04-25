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
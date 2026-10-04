[![Build and Test](https://github.com/rust-lang-by/rust-bot/actions/workflows/build.yml/badge.svg)](https://github.com/rust-lang-by/rust-bot/actions/workflows/build.yml) [![Deploy](https://github.com/rust-lang-by/rust-bot/actions/workflows/deploy.yml/badge.svg)](https://github.com/rust-lang-by/rust-bot/actions/workflows/deploy.yml)

# rust-bot
Telegram bot triggered by rust word.


# How to Run

1. Declare the env variables

    ```$ export TELOXIDE_TOKEN=<Your token here> ```

    ```$ export DATABASE_URL=<Your postgress db url here>```


   Optional — daily Hacker News digest (disabled unless `HN_DIGEST_CHAT_IDS` is set):

   | Variable | Default | Meaning |
   |---|---|---|
   | `HN_DIGEST_CHAT_IDS` | _(unset = off)_ | Comma-separated numeric chat ids, e.g. `-1001234567890,-1009876543210`. The bot must be able to post there (admin with "Post messages" in channels). |
   | `HN_DIGEST_TIME_UTC` | `17:00` | Daily post time, `HH:MM` in UTC. |
   | `HN_DIGEST_TOP_N` | `3` | Stories per digest, `1`–`10`. |
   | `HN_DIGEST_RUN_ON_STARTUP` | `false` | Also post once at startup. For local testing only — every restart/deploy posts again. |


   On-demand digest: in any group the bot is in, send `/hn_digest [1-10]` (default `3`; `/hn-digest` also works). The bot replies with that many summarized top stories; an invalid count gets a usage hint. Works whether or not the daily digest is enabled.


2. Create the database.

    ```$ sqlx db create```


3. Run sql migrations

    ```$ sqlx migrate run```

   
4. Build and run application

```shell
 cargo run
```

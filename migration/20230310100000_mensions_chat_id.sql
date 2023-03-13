ALTER TABLE mentions
    ADD chat_id BIGINT;

UPDATE mentions
SET chat_id = -1001146107319
WHERE mentions.chat_id IS NULL;

ALTER TABLE mentions
    DROP CONSTRAINT mentions_pkey;

ALTER TABLE mentions
    ADD PRIMARY KEY (user_id, chat_id);
END

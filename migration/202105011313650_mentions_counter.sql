ALTER TABLE mentions
    ADD username VARCHAR(255);

UPDATE mentions
    SET username = 'unknown'
        WHERE username IS NULL;

ALTER TABLE mentions
    ADD counter INT DEFAULT 1;
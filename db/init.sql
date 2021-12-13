
CREATE TABLE IF NOT EXISTS users(
    id SERIAL UNIQUE PRIMARY KEY,
    username VARCHAR(15),
    passw VARCHAR(15)
);

CREATE TABLE IF NOT EXISTS feeds(
    id SERIAL UNIQUE PRIMARY KEY,
    f_name VARCHAR(30)
);

CREATE TABLE IF NOT EXISTS messages(
    feed_id SERIAL,
    usr_id SERIAL,
    id SERIAL,
    mess VARCHAR(150),
    CONSTRAINT fk_feed_id
        FOREIGN KEY(feed_id)
        REFERENCES feeds(id),
    CONSTRAINT fk_usr_id
        FOREIGN KEY(usr_id)
        REFERENCES users(id),
    PRIMARY KEY(usr_id, feed_id, id)
);
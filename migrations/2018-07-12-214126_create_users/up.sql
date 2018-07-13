CREATE TABLE users (
  id SERIAL PRIMARY KEY
  , email VARCHAR(254) UNIQUE NOT NULL
  , uuid VARCHAR(32) UNIQUE NOT NULL
  , password VARCHAR(60) NOT NULL
  , name VARCHAR(100)
  , subscription_expires DATE
);

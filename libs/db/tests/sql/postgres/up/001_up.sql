CREATE TABLE IF NOT EXISTS people(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    "name" TEXT NOT NULL,
    age INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS addresses(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    region TEXT NOT NULL,
    settlement TEXT NOT NULL,
    street TEXT NOT NULL,
    location_designation TEXT NOT NULL,
    lon_coord INT,
    lat_coord INT
);
CREATE TABLE IF NOT EXISTS people_addresses_rel(
    people_id BIGINT NOT NULL,
    addresses_id BIGINT NOT NULL,
    PRIMARY KEY (people_id, addresses_id)
)
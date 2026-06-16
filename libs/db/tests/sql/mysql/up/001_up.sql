CREATE TABLE IF NOT EXISTS people(
    `id` INTEGER NOT NULL AUTO_INCREMENT,
    `name` TEXT NOT NULL,
    `age` INTEGER NOT NULL,
    PRIMARY KEY (id)
);
CREATE TABLE IF NOT EXISTS addresses(
    `id` INTEGER NOT NULL AUTO_INCREMENT,
    `region` TEXT NOT NULL,
    `settlement` TEXT NOT NULL,
    `street` TEXT NOT NULL,
    `location_designation` TEXT NOT NULL,
    `lon_coord` INT,
    `lat_coord` INT,
    PRIMARY KEY (id)
);
CREATE TABLE IF NOT EXISTS people_addresses_rel(
    `people_id` BIGINT NOT NULL,
    `addresses_id` BIGINT NOT NULL,
    PRIMARY KEY (people_id, addresses_id)
)
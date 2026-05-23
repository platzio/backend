ALTER TABLE helm_registries
    ADD COLUMN provider VARCHAR NOT NULL DEFAULT 'Ecr';

create table user_tokens(
    id uuid not null primary key,
    user_id uuid not null references users(id) on delete cascade,
    created_at timestamptz not null default now(),
    secret_hash varchar not null
);

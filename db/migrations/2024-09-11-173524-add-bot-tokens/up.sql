create table bot_tokens(
    id uuid not null primary key,
    bot_id uuid not null references bots(id) on delete cascade,
    created_at timestamptz not null default now(),
    created_by_user_id uuid not null references users(id),
    secret_hash varchar not null
);

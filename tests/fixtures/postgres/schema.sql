-- Schema previously created by the (now-removed) migration system.
-- Applied as the first fixture so subsequent data fixtures have tables to populate.

create extension if not exists "uuid-ossp";

create table if not exists "user"
(
    user_id  uuid primary key default uuid_generate_v1mc(),
    username text unique not null
);

create table if not exists post (
    post_id uuid primary key default uuid_generate_v1mc(),
    user_id uuid not null references "user"(user_id),
    content text not null,
    created_at timestamptz default now()
);

create index if not exists post_created_at_idx on post(created_at desc);

create table if not exists comment (
    comment_id uuid primary key default uuid_generate_v1mc(),
    post_id uuid not null references post(post_id),
    user_id uuid not null references "user"(user_id),
    content text not null,
    created_at timestamptz not null default now()
);

create index if not exists comment_created_at_idx on comment(created_at desc);

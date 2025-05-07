CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS rooms(
    id uuid PRIMARY KEY,
    code character(6) NOT NULL UNIQUE,
    kind smallint NOT NULL,
    open_time timestamp NOT NULL,
    close_time timestamp NOT NULL
);

CREATE TABLE IF NOT EXISTS teams(
    id uuid PRIMARY KEY,
    room_id uuid NOT NULL references rooms(id) ON DELETE CASCADE,
    name character varying(128) NOT NULL,
    penalty int NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS accounts(
    id uuid PRIMARY KEY,
    team_id uuid NOT NULL references teams(id) ON DELETE CASCADE,
    full_name character varying(64) NOT NULL,
    student_id character varying(16) NOT NULL,
    email character varying(64) NOT NULL UNIQUE,
    password character varying(128) NOT NULL,
    phone character varying(12) NOT NULL,
    school character varying(128) NOT NULL,
    dob date NOT NULL,
    is_banned boolean NOT NULL DEFAULT false
);

CREATE TABLE IF NOT EXISTS questions(
    id uuid PRIMARY KEY,
    room_id uuid NOT NULl references rooms(id) ON DELETE CASCADE,
    description_path text NOT NULL,
    score real NOT NULL,
    time_limit int, -- millisecond, only exist for BE
    memory_limit int -- megabyte, only exist for BE
);

CREATE TABLE IF NOT EXISTS test_cases(
    id uuid PRIMARY KEY,
    question_id uuid NOT NULL references questions(id) ON DELETE CASCADE,
    index int NOT NULL,
    input_path character varying(128),
    output_path character varying(128) NOT NULL,
    is_hidden boolean NOT NULL
);

CREATE TABLE IF NOT EXISTS submissions(
    id uuid PRIMARY KEY,
    question_id uuid NOT NULL references questions(id) ON DELETE CASCADE,
    team_id uuid NOT NULL references teams(id) ON DELETE CASCADE,
    language smallint NOT NULL,
    code text NOT NULL,
    score real,
    failed_test_case int,
    error text,
    created_at timestamp NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS submission_details(
    id uuid PRIMARY KEY,
    submission_id uuid NOT NULL references submissions(id) ON DELETE CASCADE,
    index int NOT NULL,
    status int NOT NULL,
    run_time int NOT NULL, -- millisecond
    stdout text NOT NULL,
    stderr text NOT NULL
);

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE room_kind AS ENUM(
    'FE',
    'BE'
);

CREATE TABLE IF NOT EXISTS rooms(
    id serial PRIMARY KEY,
    code character(12) NOT NULL UNIQUE,
    kind room_kind NOT NULL,
    open_time timestamp NOT NULL,
    close_time timestamp NOT NULL,
    created_at timestamp NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS teams(
    id serial PRIMARY KEY,
    room_id serial NOT NULL references rooms(id),
    name character varying(128) NOT NULL,
    total_score real NOT NULL DEFAULT 0,
    penalty int NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS accounts(
    id uuid PRIMARY KEY,
    team_id serial NOT NULL references teams(id),
    full_name character varying(64) NOT NULL,
    student_id character varying(16) NOT NULL,
    email character varying(64) NOT NULL UNIQUE,
    password character varying(128) NOT NULL,
    phone character varying(12) NOT NULL,
    school character varying(128) NOT NULL,
    dob date NOT NULL,
    is_banned boolean NOT NULL DEFAULT false,
    created_at timestamp NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS questions(
    id uuid PRIMARY KEY,
    room_id serial NOT NULl references rooms(id),
    score int NOT NULL,
    timeout int NOT NULL
);

CREATE TABLE IF NOT EXISTS test_cases(
    id uuid PRIMARY KEY,
    question_id uuid NOT NULL references questions(id),
    input_path character varying(64) NOT NULL,
    output_path character varying(64) NOT NULL,
    is_hidden boolean NOT NULL
);

CREATE TYPE language AS ENUM(
    'CPP',
    'JAVA',
    'PYTHON',
    'HTML'
);

CREATE TABLE IF NOT EXISTS submission_histories(
    id uuid PRIMARY KEY,
    question_id uuid NOT NULL references questions(id),
    team_id serial NOT NULL references teams(id),
    language language NOT NULL,
    code text NOT NULL,
    score real,
    compilation_error text,
    created_at timestamp NOT NULL DEFAULT now()
);

CREATE TYPE status AS ENUM(
    'TIMEOUT',
    'FAILED',
    'SUCCESS'
);

CREATE TABLE IF NOT EXISTS submission_details(
    submission_history_id uuid NOT NULL references submission_histories(id),
    test_case_id uuid NOT NULL references test_cases(id),
    status status NOT NULL,
    run_time int NOT NULL,
    stdout text,
    stderr text,

    PRIMARY KEY (submission_history_id, test_case_id)
);

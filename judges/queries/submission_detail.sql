--! insert
INSERT INTO submission_details(
    submission_id,
    index,
    status,
    run_time,
    stdout,
    stderr
) VALUES (
    :submission_id,
    :index,
    :status,
    :run_time,
    :stdout,
    :stderr
);

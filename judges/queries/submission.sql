--! get_submission
SELECT question_id, language, code
FROM submissions
WHERE id = :id;

--! update_submission_status
UPDATE submissions
SET
    score = :score,
    error = :error,
    failed_test_case = :failed_test_case
WHERE id = :id;

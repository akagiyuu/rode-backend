--! get
SELECT question_id, language, code
FROM submissions
WHERE id = :id;

--! update_status (error?, failed_test_case?)
UPDATE submissions
SET
    score = :score,
    error = :error,
    failed_test_case = :failed_test_case
WHERE id = :id;

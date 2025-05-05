--! get_submission
SELECT question_id, language, code
FROM submissions
WHERE id = :id;

--! update_submission_status
UPDATE 

--! get_by_question_id
SELECT id, input_path, output_path, is_hidden
FROM test_cases
WHERE question_id = :question_id;

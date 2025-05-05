--! get : (time_limit?, memory_limit?)
SELECT score, time_limit, memory_limit
FROM questions
WHERE id = :id;

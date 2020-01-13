SELECT invocations.* FROM invocations
INNER JOIN runs
ON
   invocations.run_id = runs.id
WHERE runs.id = $1
ORDER BY invocations.id DESC
LIMIT 1

<?php
// phase6-diff: id=PHASE6_JSON_BASICS area=json-pcre-date expect=pass
$data = ["phase6", true, [1, 2, 3]];
$json = json_encode($data);
echo $json, "\n";
$decoded = json_decode($json, true);
echo $decoded[0], "|", count($decoded[2]), "|", ($decoded[1] ? "true" : "false"), "\n";
echo json_last_error(), "|", json_last_error_msg(), "\n";

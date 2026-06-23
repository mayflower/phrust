<?php
// phase6-diff: id=PHASE6_CORPUS_JSON_CONFIG area=corpus expect=pass
// purpose: Local JSON config decode, nested array reads, and stable re-encoding.
// reference-output:
// routes=home,user
// page=25
// {"debug":true,"first":"home"}
$config = json_decode(file_get_contents(__DIR__ . '/config/app.json'), true);
echo 'routes=', implode(',', $config['routes']), "\n";
echo 'page=', $config['limits']['page'], "\n";
echo json_encode(array('debug' => $config['debug'], 'first' => $config['routes'][0])), "\n";

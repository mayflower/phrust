<?php
// phase6-diff: id=PHASE6_CORPUS_ARRAY_MANIPULATION area=corpus expect=pass
// purpose: Request option normalization with map/filter/merge/sort style array operations.
// reference-output:
// alpha,beta
// {"cache":false,"debug":true,"limit":10}
$input = array('debug' => '1', 'cache' => '0', 'limit' => '10');
$normalized = array(
    'cache' => $input['cache'] === '1',
    'debug' => $input['debug'] === '1',
    'limit' => intval($input['limit']),
);
$names = array_filter(array('alpha', '', 'beta'));
sort($names);

echo implode(',', $names), "\n";
echo json_encode($normalized), "\n";

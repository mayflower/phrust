<?php
// phase6-diff: id=PHASE6_CORPUS_ENV_PARSING area=corpus expect=pass
// purpose: Framework-style environment parsing with deterministic request-local getenv/putenv.
// reference-output:
// dev
// debug-on
// workers=3
putenv('APP_ENV=dev');
putenv('APP_DEBUG=1');
putenv('APP_WORKERS=3');

$debug = getenv('APP_DEBUG') === '1';
$workers = intval(getenv('APP_WORKERS'));

echo getenv('APP_ENV'), "\n";
echo $debug ? "debug-on\n" : "debug-off\n";
echo 'workers=', $workers, "\n";

<?php
// phase6-diff: id=PHASE6_STDLIB_INI_CONFIG area=stdlib expect=pass
ini_set('memory_limit', '64M');
echo ini_get('memory_limit'), "\n";
echo ini_get('missing.option') === false ? "missing\n" : "bad\n";
echo get_cfg_var('display_errors'), "\n";
ini_set('include_path', 'phase6-lib');
$flat = ini_get_all(null, false);
echo $flat['include_path'], "\n";
$details = ini_get_all();
echo $details['include_path']['local_value'], "|", $details['include_path']['access'], "\n";

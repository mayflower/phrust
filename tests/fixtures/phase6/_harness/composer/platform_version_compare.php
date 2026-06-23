<?php
// phase6-diff: id=PHASE6_COMPOSER_VERSION_COMPARE area=composer expect=pass
$cases = array(
    array('8.5.7', '8.5.0', null),
    array('8.5.7', '8.5.7', 'eq'),
    array('8.5.7-dev', '8.5.7', '<'),
    array('8.5.7RC1', '8.5.7', 'lt'),
    array('8.5.7pl1', '8.5.7', 'gt'),
    array('8.5.7', '8.5.7', '>='),
    array('8.5.7', '8.5.8', '<>'),
);

foreach ($cases as $case) {
    if ($case[2] === null) {
        echo version_compare($case[0], $case[1]), "\n";
    } else {
        echo version_compare($case[0], $case[1], $case[2]) ? "true\n" : "false\n";
    }
}

<?php
// stdlib-diff: id=STDLIB_CORPUS_REGEX_ROUTING area=corpus expect=pass
// purpose: Router-like PCRE path match, numeric captures, and dispatch table lookup.
// reference-output:
// route=user
// id=42
$path = '/users/42';
$routes = array('user' => '/^\\/users\\/([0-9]+)$/');

foreach ($routes as $name => $pattern) {
    if (preg_match($pattern, $path)) {
        echo 'route=', $name, "\n";
        echo 'id=', substr($path, 7), "\n";
    }
}

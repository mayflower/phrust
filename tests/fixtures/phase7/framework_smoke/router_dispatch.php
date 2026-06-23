<?php
$path = '/users/42';
$routes = array(
    'home' => '/^\\/$/',
    'user' => '/^\\/users\\/([0-9]+)$/',
);

foreach ($routes as $name => $pattern) {
    if (preg_match($pattern, $path)) {
        echo 'route=', $name, "\n";
        echo 'id=', substr($path, 7), "\n";
    }
}

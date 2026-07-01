<?php
require __DIR__ . '/wp-includes/hooks.php';
require __DIR__ . '/wp-includes/autoload.php';

spl_autoload_register(function ($class) {
    $prefix = 'Synthetic\\';
    if (strpos($class, $prefix) !== 0) {
        return;
    }
    $relative = str_replace('\\', '/', substr($class, strlen($prefix)));
    require __DIR__ . '/wp-includes/classes/' . $relative . '.php';
});

add_action('init', function () {
    $controller = new Synthetic\Controller();
    $controller->render();
});

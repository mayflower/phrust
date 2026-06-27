<?php
$items = array();
for ($i = 1; $i <= 3; $i++) {
    $items[] = array(
        'id' => $i,
        'slug' => 'item-' . $i,
    );
}

$response = array(
    'ok' => true,
    'count' => count($items),
    'items' => $items,
);

echo json_encode($response), "\n";

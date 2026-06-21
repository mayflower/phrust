<?php

$prefix = 'id-';
$format = static fn (int $id): string => $prefix . $id;

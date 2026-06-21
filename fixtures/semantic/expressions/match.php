<?php

namespace Exprs;

$label = match ($value) {
    1, 2 => "small",
    3 => "three",
    default => "other",
};

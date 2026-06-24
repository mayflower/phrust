<?php
function perf_cache_inc($value) {
    return $value + 1;
}

echo perf_cache_inc(4), "\n";

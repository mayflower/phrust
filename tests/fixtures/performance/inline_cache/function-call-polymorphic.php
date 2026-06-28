<?php
function perf_ic_poly_a() {
    return "A";
}

function perf_ic_poly_b() {
    return "B";
}

function perf_ic_poly_c() {
    return "C";
}

function perf_ic_poly_d() {
    return "D";
}

function perf_ic_poly_e() {
    return "E";
}

$out = "";
foreach (["perf_ic_poly_a", "perf_ic_poly_b", "perf_ic_poly_a", "perf_ic_poly_b"] as $name) {
    $out .= $name();
}
foreach (["perf_ic_poly_a", "perf_ic_poly_b", "perf_ic_poly_c", "perf_ic_poly_d", "perf_ic_poly_e", "perf_ic_poly_a"] as $name) {
    $out .= $name();
}

echo "call-poly:", $out, "\n";

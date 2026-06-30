<?php
const DEFAULT_LABEL = "B";

class DefaultsSource {
    public const FIRST = "A";
}

function defaults_matrix(
    $items = ["left" => DefaultsSource::FIRST, "right" => DEFAULT_LABEL],
    $selected = ["x", "y"][1],
    $fallback = null ?? "fallback",
    $conditional = true ? "yes" : "no",
) {
    echo $items["left"], "|", $items["right"], "|", $selected, "|", $fallback, "|", $conditional;
}

defaults_matrix();

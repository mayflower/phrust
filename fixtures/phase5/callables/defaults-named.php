<?php
function defaults_named($first, $second = "B", $third = "C") {
    echo $first, "|", $second, "|", $third;
}

defaults_named(third: "Z", first: "A");

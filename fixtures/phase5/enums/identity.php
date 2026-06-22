<?php
// phase5-runtime: category=enums expect=pass
enum Direction {
    case North;
    case South;
}

if (Direction::North === Direction::North) {
    echo "same\n";
} else {
    echo "different\n";
}

if (Direction::North === Direction::South) {
    echo "bad\n";
} else {
    echo "different\n";
}

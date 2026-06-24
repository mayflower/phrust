<?php
// runtime-semantics: category=enums expect=pass
enum State {
    case Ready;
    case Done;
}

$state = State::Done;
echo match ($state) {
    State::Ready => "ready",
    State::Done => "done",
};

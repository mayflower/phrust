<?php
function text(): string
{
    return "ok";
}

function number(): int
{
    return 4;
}

function nothing(): void
{
    return;
}

echo text(), "|", number(), "|";
echo nothing(), "x\n";

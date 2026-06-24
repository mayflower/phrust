<?php
// runtime-fixture: kind=pass id=unit-enum
enum RuntimeStatusGap
{
    case Ready;
}

echo RuntimeStatusGap::Ready->name;

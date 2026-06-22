<?php
// phase5-runtime: category=enums expect=fail
enum Size: string {
    case Small = 's';
}

Size::from('missing');

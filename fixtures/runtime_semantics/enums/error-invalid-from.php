<?php
// runtime-semantics: category=enums expect=fail
enum Size: string {
    case Small = 's';
}

Size::from('missing');

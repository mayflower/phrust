<?php
// phase5-runtime: category=enums expect=pass
enum Token: string {
    case A = 'a';
    public const LABEL = 'token';

    public function label(): string { return self::LABEL . ':' . $this->value; }
    public static function first(): self { return self::A; }
}

echo Token::A->label() . "\n";
if (Token::first() === Token::A) {
    echo "same\n";
} else {
    echo "bad\n";
}

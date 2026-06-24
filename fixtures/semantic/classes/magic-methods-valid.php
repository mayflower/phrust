<?php

class MagicValidFixture
{
    public function __construct(string $name)
    {
    }

    public function __destruct()
    {
    }

    public function __call(string $name, array $arguments): mixed
    {
        return null;
    }

    public static function __callStatic(string $name, array $arguments): mixed
    {
        return null;
    }

    public function __get(string $name): mixed
    {
        return null;
    }

    public function __set(string $name, mixed $value): void
    {
    }

    public function __isset(string $name): bool
    {
        return false;
    }

    public function __unset(string $name): void
    {
    }

    public function __sleep(): array
    {
        return [];
    }

    public function __wakeup(): void
    {
    }

    public function __serialize(): array
    {
        return [];
    }

    public function __unserialize(array $data): void
    {
    }

    public function __toString(): string
    {
        return '';
    }

    public function __invoke(): void
    {
    }

    public static function __set_state(array $properties): object
    {
        return new self('restored');
    }

    public function __clone()
    {
    }

    public function __debugInfo(): ?array
    {
        return [];
    }
}

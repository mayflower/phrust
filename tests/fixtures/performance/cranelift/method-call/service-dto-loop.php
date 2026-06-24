<?php
class PerfDirectDto {
    public int $base;

    public function __construct(int $base) {
        $this->base = $base;
    }
}

class PerfDirectService {
    public function score(PerfDirectDto $dto, int $x): int {
        return $dto->base + $x + 1;
    }
}

$service = new PerfDirectService();
$dto = new PerfDirectDto(7);
$sum = 0;
for ($i = 0; $i < 24; $i++) {
    $sum = $sum + $service->score($dto, $i);
}
echo $sum, "\n";

<?php
class Phase7DirectDto {
    public int $base;

    public function __construct(int $base) {
        $this->base = $base;
    }
}

class Phase7DirectService {
    public function score(Phase7DirectDto $dto, int $x): int {
        return $dto->base + $x + 1;
    }
}

$service = new Phase7DirectService();
$dto = new Phase7DirectDto(7);
$sum = 0;
for ($i = 0; $i < 24; $i++) {
    $sum = $sum + $service->score($dto, $i);
}
echo $sum, "\n";

--TEST--
imagick: class surface and ImageMagick backend gate
--EXTENSIONS--
imagick
--FILE--
<?php
echo extension_loaded('imagick') ? "loaded\n" : "missing\n";
foreach (['Imagick', 'ImagickDraw', 'ImagickPixel', 'ImagickPixelIterator', 'ImagickException'] as $class) {
    echo class_exists($class, false) ? "$class class\n" : "$class missing\n";
}
$class = new ReflectionClass('Imagick');
echo $class->getName(), "|", $class->getExtensionName(), "|", ($class->isInternal() ? "internal" : "user"), "\n";
foreach (['readImage', 'writeImage', 'resizeImage', 'cropImage', 'thumbnailImage', 'identifyImage'] as $method) {
    echo method_exists('Imagick', $method) ? "$method method\n" : "$method missing\n";
}
new Imagick();
?>
--EXPECTF--
loaded
Imagick class
ImagickDraw class
ImagickPixel class
ImagickPixelIterator class
ImagickException class
Imagick|imagick|internal
readImage method
writeImage method
resizeImage method
cropImage method
thumbnailImage method
identifyImage method
%s: runtime-diagnostic: %s"E_PHP_VM_UNSUPPORTED_IMAGICK"%sImageMagick backend capability gate%s
%s: runtime_error: E_PHP_VM_UNSUPPORTED_IMAGICK: class %s requires an ImageMagick backend capability gate

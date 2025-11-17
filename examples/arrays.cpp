#include "arrays.hpp"
namespace arrays {


int main () {
int* a = new int[2 * 8]{0, 0};
int i = 0;
while (i < 16) {
a[i] = i;
i += 1;
}
println(i32_to_str(a[15]));
return 0;
}
}
int main() {
return arrays::main();
}

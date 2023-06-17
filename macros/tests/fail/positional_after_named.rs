fn main() {
    sqlm_macros::sql!("{one} {}", one = 1, 2);
}

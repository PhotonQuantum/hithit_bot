#[macro_export]
macro_rules! bail {
    ($ee: expr, $match: pat) => {
        if matches!($ee, $match) {
            return;
        } else {
            $ee
        }
    };
    (unwrap $ee: expr, $match: pat) => {
        if matches!($ee, $match) {
            return;
        } else {
            $ee.unwrap()
        }
    };
}

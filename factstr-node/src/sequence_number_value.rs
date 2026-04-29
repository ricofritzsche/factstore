use napi::Error;
use napi::bindgen_prelude::BigInt;

pub fn bigint_from_u64(value: u64) -> BigInt {
    value.into()
}

pub fn bigint_to_u64(value: BigInt) -> napi::Result<u64> {
    let (sign_bit, unsigned_value, lossless) = value.get_u64();
    if sign_bit || !lossless {
        return Err(Error::from_reason(
            "FACTSTR sequence and context values must be non-negative lossless BigInt values"
                .to_owned(),
        ));
    }

    Ok(unsigned_value)
}

pub fn option_bigint_from_u64(value: Option<u64>) -> Option<BigInt> {
    value.map(bigint_from_u64)
}

pub fn option_bigint_to_u64(value: Option<BigInt>) -> napi::Result<Option<u64>> {
    value.map(bigint_to_u64).transpose()
}

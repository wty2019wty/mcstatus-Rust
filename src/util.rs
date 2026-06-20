use std::future::Future;

use crate::error::{McStatusError, Result};

/// Returns the first non-None argument.
///
/// Unlike `Option::or`, this treats falsy-like values (0, empty string, etc.)
/// as valid values and only skips `None`.
///
/// This is equivalent to Python's `or_none(*args)` utility.
pub fn or_none<T>(a: Option<T>, b: Option<T>) -> Option<T> {
    a.or(b)
}

/// Retry an async operation up to `tries` times.
///
/// If the operation succeeds, the result is returned immediately.
/// If it fails, it is retried up to `tries` times. After the last
/// failure, the error is returned.
pub async fn retry<F, Fut, T>(tries: usize, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..tries {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                last_error = Some(e);
                if attempt < tries - 1 {
                    // Small delay between retries could be added here
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        McStatusError::Other("Retry exhausted without any attempts".into())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_or_none() {
        assert_eq!(or_none(Some(1), None), Some(1));
        assert_eq!(or_none(None, Some(2)), Some(2));
        assert_eq!(or_none(Some(0), None), Some(0)); // 0 is valid
        assert_eq!(or_none(None, None), None);
    }

    #[tokio::test]
    async fn test_retry_success_first_try() {
        let result = retry(3, || async { Ok(42i32) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let result: Result<i32> = retry(3, || {
            let c = c.clone();
            async move {
                let attempts = c.fetch_add(1, Ordering::SeqCst);
                if attempts < 2 {
                    Err(McStatusError::Other("fail".into()))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3); // tried 3 times
    }

    #[tokio::test]
    async fn test_retry_all_fail() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let result: Result<i32> = retry(3, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err(McStatusError::Other("fail".into()))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}

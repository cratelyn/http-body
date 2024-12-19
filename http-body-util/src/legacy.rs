//! XXX(kate): upgrade adaptor documentation.

use bytes::Buf;
use futures_util::ready;
use http::HeaderMap;
use http_body::{Body, Frame, SizeHint};
use pin_project_lite::pin_project;

use std::pin::Pin;
use std::task::{Context, Poll};

/// Trait representing a streaming body of a Request or Response.
///
/// Data is streamed via the `poll_data` function, which asynchronously yields `T: Buf` values. The
/// `size_hint` function provides insight into the total number of bytes that will be streamed.
///
/// The `poll_trailers` function returns an optional set of trailers used to finalize the request /
/// response exchange. This is mostly used when using the HTTP/2.0 protocol.
// XXX(kate): document this.
pub trait LegacyBody {
    /// Values yielded by the `Body`.
    type Data: Buf;

    /// The error type this `Body` might generate.
    type Error;

    /// Attempt to pull out the next data buffer of this stream.
    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>>;

    /// Poll for an optional **single** `HeaderMap` of trailers.
    ///
    /// This function should only be called once `poll_data` returns `None`.
    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>>;

    /// Returns `true` when the end of stream has been reached.
    ///
    /// An end of stream means that both `poll_data` and `poll_trailers` will
    /// return `None`.
    ///
    /// A return value of `false` **does not** guarantee that a value will be
    /// returned from `poll_stream` or `poll_trailers`.
    fn is_end_stream(&self) -> bool {
        false
    }

    /// Returns the bounds on the remaining length of the stream.
    ///
    /// When the **exact** remaining length of the stream is known, the upper bound will be set and
    /// will equal the lower bound.
    fn size_hint(&self) -> SizeHint {
        SizeHint::default()
    }
}

pin_project! {
    /// Wraps a legacy v0.4 asynchronous request or response body.
    pub struct UpgradeBody<B> {
        #[pin]
        inner: B,
        data_finished: bool,
        stream_finished: bool,
    }
}

// === impl UpgradeBody ===

impl<B> UpgradeBody<B>
where
    B: LegacyBody,
{
    /// Returns a legacy adaptor body.
    pub fn new(body: B) -> Self {
        let empty = body.is_end_stream();
        Self {
            inner: body,
            // These two flags should be true if the inner body is already exhausted.
            data_finished: empty,
            stream_finished: empty,
        }
    }
}

/// A [`UpgradeBody`] implements the v1.0 [`Body`][body] interface.
///
/// [body]: http_body::Body
impl<B> Body for UpgradeBody<B>
where
    B: LegacyBody,
{
    type Data = B::Data;
    type Error = B::Error;
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<<Self as Body>::Data>, Self::Error>>> {
        let this = self.project();
        let inner = this.inner;
        let data_finished = this.data_finished;
        let stream_finished = this.stream_finished;

        // If the stream is finished, return `None`.
        if *stream_finished {
            return Poll::Ready(None);
        }

        // If all of the body's data has been polled, it is time to poll the trailers.
        if *data_finished {
            let trailers = ready!(inner.poll_trailers(cx))
                .map(|o| o.map(Frame::trailers))
                .transpose();
            // Mark this stream as finished once the inner body yields a result.
            *stream_finished = true;
            return Poll::Ready(trailers);
        }

        // If we're here, we should poll the inner body for a chunk of data.
        let data = ready!(inner.poll_data(cx)).map(|r| r.map(Frame::data));
        if data.is_none() {
            // If `None` was yielded, mark the data as finished.
            *data_finished = true;
        } else if matches!(data, Some(Err(_))) {
            // If an error was yielded, the data is finished *and* the stream is finished.
            *data_finished = true;
            *stream_finished = true;
        }

        Poll::Ready(data)
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.stream_finished
    }

    #[inline]
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

// XXX(kate); write tests

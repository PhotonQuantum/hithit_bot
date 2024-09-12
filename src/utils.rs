use color_eyre::Handler;
use eyre::Report;
use sentry::protocol::Event;
use tracing::field::Empty;

pub fn sentry_capture<T, E>(r: Result<T, E>) -> Result<T, Report>
where
    E: Into<Report>,
{
    match r {
        Ok(r) => Ok(r),
        Err(e) => {
            let report = e.into();
            tracing::error!(sentry_ignore = Empty, "{:?}", report);
            if let Some(event) = event_from_report(&report) {
                sentry::capture_event(event);
            }
            Err(report)
        }
    }
}

fn event_from_report(r: &Report) -> Option<Event<'static>> {
    let err: &dyn std::error::Error = r.as_ref();
    let handler: &Handler = r.handler().downcast_ref()?;

    let mut event = sentry::event_from_error(err);
    let exc = event.exception.iter_mut().last()?;

    if let Some(backtrace) = handler.backtrace() {
        exc.stacktrace =
            sentry::integrations::backtrace::parse_stacktrace(&format!("{backtrace:#?}"));
    }

    Some(event)
}

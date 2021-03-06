//! The server-side WebSocket request.

use server::Response;
use result::{WebSocketResult, WebSocketError};
use header::{WebSocketKey, WebSocketVersion, WebSocketProtocol, WebSocketExtensions, Origin};

pub use hyper::uri::RequestUri;

use hyper::version::HttpVersion;
use hyper::status::StatusCode;
use hyper::header::Headers;
use hyper::header::{Connection, ConnectionOption};
use hyper::header::{Upgrade, Protocol};
use hyper::http::read_request_line;
use hyper::method::Method;

use unicase::UniCase;

/// Represents a server-side (incoming) request.
pub struct Request<R: Reader, W: Writer> {
	/// The target URI for this request.
    pub url: RequestUri,
	
    /// The HTTP version of this request.
    pub version: HttpVersion,
	
	/// The headers of this request.
	pub headers: Headers,
	
	reader: R,
	writer: W,
}

impl<R: Reader, W: Writer> Request<R, W> {
	/// Short-cut to obtain the WebSocketKey value.
	pub fn key(&self) -> Option<&WebSocketKey> {
		self.headers.get()
	}
	/// Short-cut to obtain the WebSocketVersion value.
	pub fn version(&self) -> Option<&WebSocketVersion> {
		self.headers.get()
	}
	/// Short-cut to obtain the WebSocketProtocol value.
	pub fn protocol(&self) -> Option<&WebSocketProtocol> {
		self.headers.get()
	}
	/// Short-cut to obtain the WebSocketExtensions value.
	pub fn extensions(&self) -> Option<&WebSocketExtensions> {
		self.headers.get()
	}
	/// Short-cut to obtain the Origin value.
	pub fn origin(&self) -> Option<&Origin> {
		self.headers.get()
	}
	/// Returns a reference to the inner Reader.
	pub fn get_reader(&self) -> &R {
		&self.reader
	}
	/// Returns a reference to the inner Writer.
	pub fn get_writer(&self) -> &W {
		&self.writer
	}
	/// Returns a mutable reference to the inner Reader.
	pub fn get_mut_reader(&mut self) -> &mut R {
		&mut self.reader
	}
	/// Returns a mutable reference to the inner Writer.
	pub fn get_mut_writer(&mut self) -> &mut W {
		&mut self.writer
	}
	/// Return the inner Reader and Writer
	pub fn into_inner(self) -> (R, W) {
		(self.reader, self.writer)
	}
	/// Reads an inbound request.
	/// 
	/// This method is used within servers, and returns an inbound WebSocketRequest.
	/// An error will be returned if the request cannot be read, or is not a valid HTTP request.
	pub fn read(reader: R, writer: W) -> WebSocketResult<Request<R, W>> {
		let mut reader = reader;
		let (method, uri, version) = try!(read_request_line(&mut reader));
		
		match method {
			Method::Get => { },
			_ => { return Err(WebSocketError::RequestError("Request method must be GET".to_string())); }
		}
		
        let headers = try!(Headers::from_raw(&mut reader));
		
		Ok(Request {
			url: uri,
			version: version,
			headers: headers,
			reader: reader,
			writer: writer,
		})
	}
	/// Check if this constitutes a valid request.
	///
	/// Note that `accept()` calls this function internally, however this may be useful for handling bad requests
	/// in a custom way.
	pub fn validate(&self) -> WebSocketResult<()> {
		if self.version == HttpVersion::Http09 || self.version == HttpVersion::Http10 {
			return Err(WebSocketError::RequestError("Unsupported request HTTP version".to_string()));
		}
		
		if self.version() != Some(&(WebSocketVersion::WebSocket13)) {
			return Err(WebSocketError::RequestError("Unsupported WebSocket version".to_string()));
		}
		
		if self.key().is_none() {
			return Err(WebSocketError::RequestError("Missing Sec-WebSocket-Key header".to_string()));
		}
		
		match self.headers.get() {
			Some(&Upgrade(ref upgrade)) => {
				if !upgrade.contains(&(Protocol::WebSocket)) {
					return Err(WebSocketError::RequestError("Invalid Upgrade WebSocket header".to_string()));
				}
			}
			None => { return Err(WebSocketError::RequestError("Missing Upgrade WebSocket header".to_string())); }
		}
		
		match self.headers.get() {
			Some(&Connection(ref connection)) => {
				if !connection.contains(&(ConnectionOption::ConnectionHeader(UniCase("Upgrade".to_string())))) {
					return Err(WebSocketError::RequestError("Invalid Connection WebSocket header".to_string()));
				}
			}
			None => { return Err(WebSocketError::RequestError("Missing Connection WebSocket header".to_string())); }
		}
		
		Ok(())
	}
	
	/// Accept this request, ready to send a response.
	///
	/// This function calls `validate()` on the request, and if the request is found to be invalid,
	/// generates a response with a Bad Request status code.
	pub fn accept(self) -> Response<R, W> {
		match self.validate() {
			Ok(()) => { }
			Err(_) => { return self.fail(); }
		}
		Response::new(self)
	}
	
	/// Fail this request by generating a Bad Request response
	pub fn fail(self) -> Response<R, W> {
		let mut response = Response::new(self);
		response.status = StatusCode::BadRequest;
		response.headers = Headers::new();
		response
	}
}
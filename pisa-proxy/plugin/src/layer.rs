// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Thanks to <https://github.com/tower-rs/tower>

/// Layer is a wrapper for service, which can have multiple different plugins
/// `S` can be of any type
pub trait Layer<S> {
    // the wrapped service
    type Service;
    // wrap a service, return a new wrapped service
    fn layer(&self, inner: S) -> Self::Service;
}

/// `Service` refers to the service that allows the
/// execution of plug-ins in pisa, or is a callback function
/// `Input` as any type of input for service
pub trait Service<Input> {
    // the service output
    type Output;
    // the service error
    type Error;

    fn handle(&mut self, input: Input) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone)]
/// Two middleware are linked together.
pub struct LayerTrans<I, O> {
    pub input: I,
    pub output: O,
}

impl<S, I, O> Layer<S> for LayerTrans<I, O>
where
    I: Layer<S>,
    O: Layer<I::Service>,
{
    type Service = O::Service;

    fn layer(&self, service: S) -> Self::Service {
        let input = self.input.layer(service);
        self.output.layer(input)
    }
}

#[derive(Clone)]
pub struct Empty;

/// Wrap `Empty Service`
impl<S> Layer<S> for Empty {
    type Service = S;

    fn layer(&self, inner: S) -> Self::Service {
        inner
    }
}

#[derive(Clone)]
pub struct ServiceBuilder<L> {
    pub layer: L,
}

impl ServiceBuilder<Empty> {
    pub fn new() -> Self {
        ServiceBuilder { layer: Empty }
    }
}

impl<L> ServiceBuilder<L> {
    // Add a wrapd service
    pub fn with_layer<T>(self, s: T) -> ServiceBuilder<LayerTrans<T, L>> {
        ServiceBuilder { layer: LayerTrans { input: s, output: self.layer } }
    }

    // wrap the service by ServiceBuilder's layers, return a new service
    pub fn build<S>(&self, s: S) -> L::Service
    where
        L: Layer<S>,
    {
        self.layer.layer(s)
    }

    // return inner service by `BoxCloneService::layer()`
    pub fn boxed_clone<S, I>(
        self,
    ) -> ServiceBuilder<
        LayerTrans<
            LayerFn<
                fn(
                    L::Service,
                ) -> BoxCloneService<
                    I,
                    <L::Service as Service<I>>::Output,
                    <L::Service as Service<I>>::Error,
                >,
            >,
            L,
        >,
    >
    where
        L: Layer<S>,
        L::Service: Service<I> + Clone + Send + 'static,
    {
        self.with_layer(BoxCloneService::layer())
    }
}

/// A `Layer` implement by closure
pub fn layer_fn<T>(f: T) -> LayerFn<T> {
    LayerFn { f }
}

#[derive(Clone)]
pub struct LayerFn<T> {
    f: T,
}

impl<T, S, L> Layer<S> for LayerFn<T>
where
    T: Fn(S) -> L,
{
    type Service = L;

    fn layer(&self, inner: S) -> Self::Service {
        (self.f)(inner)
    }
}

/// A `Service` implement by closure
pub fn service_fn<T>(f: T) -> ServiceFn<T> {
    ServiceFn { f }
}

#[derive(Clone)]
pub struct ServiceFn<T> {
    f: T,
}

impl<T, R, E, Input> Service<Input> for ServiceFn<T>
where
    T: FnMut(Input) -> Result<R, E>,
{
    type Output = R;
    type Error = E;

    fn handle(&mut self, input: Input) -> Result<Self::Output, Self::Error> {
        (self.f)(input)
    }
}

impl<'a, T, S> Layer<S> for &'a T
where
    T: ?Sized + Layer<S>,
{
    type Service = T::Service;

    fn layer(&self, inner: S) -> Self::Service {
        (**self).layer(inner)
    }
}

pub struct BoxCloneService<T, O, E>(Box<dyn CloneService<T, Output = O, Error = E> + Send>);

impl<T, O, E> BoxCloneService<T, O, E> {
    pub fn new<S>(inner: S) -> Self
    where
        S: Service<T, Output = O, Error = E> + Clone + Send + 'static,
    {
        BoxCloneService(Box::new(inner))
    }

    /// Return a `Layer` for  wrapping a `Service` in `BoxCloneService`
    pub fn layer<S>() -> LayerFn<fn(S) -> Self>
    where
        S: Service<T, Output = O, Error = E> + Clone + Send + 'static,
    {
        layer_fn(Self::new)
    }
}

// implement `Service` for `BoxCloneService`
impl<T, O, E> Service<T> for BoxCloneService<T, O, E> {
    type Output = O;
    type Error = E;

    fn handle(&mut self, input: T) -> Result<O, E> {
        self.0.handle(input)
    }
}

impl<T, O, E> Clone for BoxCloneService<T, O, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

trait CloneService<I>: Service<I> {
    fn clone_box(
        &self,
    ) -> Box<dyn CloneService<I, Output = Self::Output, Error = Self::Error> + Send>;
}

impl<I, T> CloneService<I> for T
where
    T: Service<I> + Send + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn CloneService<I, Output = T::Output, Error = T::Error> + Send> {
        Box::new(self.clone())
    }
}

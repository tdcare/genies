/*
 * @Author: tzw
 * @Date: 2021-11-29 22:16:53
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-17 02:41:10
 */
// use crate::command::HandleCommand;
// use crate::ddd::event::{DomainEventView,  Sequence};
use serde::de::DeserializeOwned;
use serde::*;
use std::fmt::*;

// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
// pub struct Generation(u64);

// impl Default for Generation {
//     fn default() -> Self {
//         Self(0)
//     }
// }

// impl Display for Generation {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

// impl Generation {
//     pub fn number(self) -> u64 {
//         self.0
//     }

//     pub fn increment(&mut self) {
//         self.0 = self.0.wrapping_add(1);
//     }
// }

// impl From<Sequence> for Generation {
//     fn from(sequence: Sequence) -> Self {
//         Self(sequence.number())
//     }
// }

// pub trait AggregateType {
//     fn aggregate_type(&self) -> String {
//         std::any::type_name::<Self>()
//     }
//     fn atype() -> String {
//         std::any::type_name::<Self>().to_string()
//     }
// }

pub trait AggregateType {
    fn aggregate_type(&self) -> String;
    fn atype() -> String;
}

pub trait WithAggregateId {
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;

    fn aggregate_id(&self) -> &Self::Id;
}

pub type AggregateIdOf<A> = <A as WithAggregateId>::Id;

// pub trait AggregateState {
//     fn generation(&self) -> Generation;
//     fn generation_mut(&mut self) -> &mut Generation;
// }

// pub trait Aggregate<E>
// where
//     E: 'static,
//     Self: 'static + Sized + WithAggregateId,
// {
//     fn apply_event(&mut self, event: DomainEventView<'_, E, Self>);

//     fn apply_all_events<'a>(
//         &mut self,
//         events: impl IntoIterator<Item = DomainEventView<'a, E, Self>>,
//     ) {
//         events.into_iter().for_each(|e| self.apply_event(e))
//     }
// }

pub trait InitializeAggregate {
    type State: WithAggregateId;

    fn initialize(aggregate_id: AggregateIdOf<Self::State>) -> Self::State;
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct VersionedAggregate<S> {
//     generation: Generation,
//     state: S,
// }

// impl<S> VersionedAggregate<S> {
//     pub fn restore(generation_number: u64, state: S) -> Self {
//         Self {
//             generation: Generation(generation_number),
//             state,
//         }
//     }

//     pub fn unwrap(self) -> S {
//         self.state
//     }

//     pub fn generation(&self) -> Generation {
//         self.generation
//     }

//     pub fn state(&self) -> &S {
//         &self.state
//     }
// }

// impl<S> InitializeAggregate for VersionedAggregate<S>
// where
//     S: WithAggregateId + InitializeAggregate<State = S>,
// {
//     type State = Self;

//     fn initialize(aggregate_id: AggregateIdOf<Self::State>) -> Self::State {
//         Self {
//             generation: Generation::default(),
//             state: S::initialize(aggregate_id),
//         }
//     }
// }

// impl<S> AggregateType for VersionedAggregate<S> {}

// impl<S> AggregateState for VersionedAggregate<S> {
//     fn generation(&self) -> Generation {
//         self.generation
//     }

//     fn generation_mut(&mut self) -> &mut Generation {
//         &mut self.generation
//     }
// }

// impl<S> WithAggregateId for VersionedAggregate<S>
// where
//     S: WithAggregateId,
// {
//     type Id = <S as WithAggregateId>::Id;

//     fn aggregate_id(&self) -> &Self::Id {
//         self.state.aggregate_id()
//     }
// }

// impl<E, S> Aggregate<E> for VersionedAggregate<S>
// where
//     E: 'static,
//     S: Aggregate<E> + AggregateType,
// {
//     fn apply_event(&mut self, event: DomainEventView<'_, E, Self>) {
//         let inner_data = event.data;
//         self.state.apply_event(event.transmute(inner_data));
//         self.generation.increment();
//     }
// }

// impl<C, S> HandleCommand<C, Self> for VersionedAggregate<S>
// where
//     S: HandleCommand<C, S>,
//     S: WithAggregateId,
// {
//     type Event = <S as HandleCommand<C, S>>::Event;
//     type Error = <S as HandleCommand<C, S>>::Error;
//     type Context = <S as HandleCommand<C, S>>::Context;

//     fn handle_command(
//         &self,
//         command: C,
//         context: &Self::Context,
//     ) -> Result<Vec<NewEvent<Self::Event, Self>>, Self::Error> {
//         self.state.handle_command(command, context).map(|events| {
//             events
//                 .into_iter()
//                 .map(|NewEvent { aggregate_id, data }| NewEvent { aggregate_id, data })
//                 .collect::<Vec<_>>()
//         })
//     }
// }

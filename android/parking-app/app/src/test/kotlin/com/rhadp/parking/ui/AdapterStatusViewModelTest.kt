package com.rhadp.parking.ui

import com.rhadp.parking.data.AdapterStateUpdate
import com.rhadp.parking.data.ServiceException
import com.rhadp.parking.data.UpdateServiceClient
import com.rhadp.parking.ui.adapter.AdapterStatusViewModel
import com.rhadp.parking.ui.adapter.AdapterStatusViewModel.UiState
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import parking.common.Common.AdapterState

/**
 * Unit tests for [AdapterStatusViewModel].
 *
 * Verifies state transitions for adapter installation monitoring:
 * - InProgress → Ready (RUNNING state)
 * - InProgress → Error (ERROR state or stream failure)
 * - Retry behavior re-starts watching
 *
 * Tests Property 5 (Navigation Integrity): Ready state on RUNNING,
 * Error state on ERROR.
 *
 * Tests Property 4 (Error Visibility): all failures produce Error state.
 *
 * Requirements: 06-REQ-2.1–2.3, 06-REQ-2.E1–2.E2, 06-REQ-7.1, 06-REQ-7.3
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AdapterStatusViewModelTest {

    private val testDispatcher = StandardTestDispatcher()

    private lateinit var updateServiceClient: UpdateServiceClient
    private lateinit var viewModel: AdapterStatusViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        updateServiceClient = mockk()
        viewModel = AdapterStatusViewModel(updateServiceClient)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // --- watchAdapter: success cases ---

    @Test
    fun `watchAdapter transitions to Ready when adapter reaches RUNNING (Property 5)`() =
        runTest {
            every { updateServiceClient.watchAdapterStates() } returns flow {
                emit(
                    AdapterStateUpdate(
                        adapterId = "a1",
                        oldState = AdapterState.ADAPTER_STATE_UNKNOWN,
                        newState = AdapterState.ADAPTER_STATE_DOWNLOADING,
                        timestamp = 1000L,
                    )
                )
                emit(
                    AdapterStateUpdate(
                        adapterId = "a1",
                        oldState = AdapterState.ADAPTER_STATE_DOWNLOADING,
                        newState = AdapterState.ADAPTER_STATE_INSTALLING,
                        timestamp = 2000L,
                    )
                )
                emit(
                    AdapterStateUpdate(
                        adapterId = "a1",
                        oldState = AdapterState.ADAPTER_STATE_INSTALLING,
                        newState = AdapterState.ADAPTER_STATE_RUNNING,
                        timestamp = 3000L,
                    )
                )
            }

            viewModel.watchAdapter("a1")
            advanceUntilIdle()

            assertEquals(UiState.Ready, viewModel.uiState.value)
        }

    @Test
    fun `watchAdapter shows intermediate state DOWNLOADING`() = runTest {
        // Use a flow that emits DOWNLOADING but doesn't complete (simulates wait)
        every { updateServiceClient.watchAdapterStates() } returns flow {
            emit(
                AdapterStateUpdate(
                    adapterId = "a1",
                    oldState = AdapterState.ADAPTER_STATE_UNKNOWN,
                    newState = AdapterState.ADAPTER_STATE_DOWNLOADING,
                    timestamp = 1000L,
                )
            )
            // Flow suspends here — simulating waiting for next event
            kotlinx.coroutines.awaitCancellation()
        }

        viewModel.watchAdapter("a1")
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected InProgress, got $state", state is UiState.InProgress)
        assertEquals("DOWNLOADING", (state as UiState.InProgress).state)
    }

    @Test
    fun `watchAdapter ignores events for other adapters`() = runTest {
        every { updateServiceClient.watchAdapterStates() } returns flow {
            // Event for a different adapter
            emit(
                AdapterStateUpdate(
                    adapterId = "other-adapter",
                    oldState = AdapterState.ADAPTER_STATE_UNKNOWN,
                    newState = AdapterState.ADAPTER_STATE_RUNNING,
                    timestamp = 1000L,
                )
            )
            // Event for our adapter
            emit(
                AdapterStateUpdate(
                    adapterId = "a1",
                    oldState = AdapterState.ADAPTER_STATE_UNKNOWN,
                    newState = AdapterState.ADAPTER_STATE_DOWNLOADING,
                    timestamp = 2000L,
                )
            )
            kotlinx.coroutines.awaitCancellation()
        }

        viewModel.watchAdapter("a1")
        advanceUntilIdle()

        val state = viewModel.uiState.value
        // Should be DOWNLOADING for a1, not Ready (which would be from other-adapter)
        assertTrue("Expected InProgress, got $state", state is UiState.InProgress)
        assertEquals("DOWNLOADING", (state as UiState.InProgress).state)
    }

    // --- watchAdapter: error cases ---

    @Test
    fun `watchAdapter transitions to Error when adapter reaches ERROR (Property 5)`() =
        runTest {
            every { updateServiceClient.watchAdapterStates() } returns flow {
                emit(
                    AdapterStateUpdate(
                        adapterId = "a1",
                        oldState = AdapterState.ADAPTER_STATE_INSTALLING,
                        newState = AdapterState.ADAPTER_STATE_ERROR,
                        timestamp = 5000L,
                    )
                )
            }

            viewModel.watchAdapter("a1")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue("Expected Error, got $state", state is UiState.Error)
            assertEquals(
                "Adapter installation failed",
                (state as UiState.Error).message,
            )
        }

    @Test
    fun `watchAdapter transitions to Error when stream fails with ServiceException`() =
        runTest {
            every { updateServiceClient.watchAdapterStates() } returns flow {
                throw ServiceException("Unable to install parking adapter")
            }

            viewModel.watchAdapter("a1")
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue("Expected Error, got $state", state is UiState.Error)
            assertEquals(
                "Unable to install parking adapter",
                (state as UiState.Error).message,
            )
        }

    @Test
    fun `watchAdapter transitions to Error on connection lost (06-REQ-2 E2)`() = runTest {
        every { updateServiceClient.watchAdapterStates() } returns flow {
            emit(
                AdapterStateUpdate(
                    adapterId = "a1",
                    oldState = AdapterState.ADAPTER_STATE_UNKNOWN,
                    newState = AdapterState.ADAPTER_STATE_DOWNLOADING,
                    timestamp = 1000L,
                )
            )
            // Simulate connection lost
            throw RuntimeException("Connection reset")
        }

        viewModel.watchAdapter("a1")
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertEquals(
            "Connection to update service lost",
            (state as UiState.Error).message,
        )
    }

    // --- retry ---

    @Test
    fun `retry re-starts watching the same adapter`() = runTest {
        // First watch: fails
        var callCount = 0
        every { updateServiceClient.watchAdapterStates() } answers {
            callCount++
            if (callCount == 1) {
                flow { throw ServiceException("Connection lost") }
            } else {
                flow {
                    emit(
                        AdapterStateUpdate(
                            adapterId = "a1",
                            oldState = AdapterState.ADAPTER_STATE_INSTALLING,
                            newState = AdapterState.ADAPTER_STATE_RUNNING,
                            timestamp = 3000L,
                        )
                    )
                }
            }
        }

        viewModel.watchAdapter("a1")
        advanceUntilIdle()

        // Should be in error state
        assertTrue(viewModel.uiState.value is UiState.Error)

        // Retry
        viewModel.retry()
        advanceUntilIdle()

        // Should now be Ready
        assertEquals(UiState.Ready, viewModel.uiState.value)
    }

    // --- initial state ---

    @Test
    fun `initial state is InProgress with INSTALLING`() {
        val state = viewModel.uiState.value
        assertTrue("Expected InProgress, got $state", state is UiState.InProgress)
        assertEquals("INSTALLING", (state as UiState.InProgress).state)
    }
}

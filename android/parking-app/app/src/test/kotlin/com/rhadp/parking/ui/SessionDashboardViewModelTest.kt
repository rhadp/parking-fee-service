package com.rhadp.parking.ui

import com.rhadp.parking.data.DataBrokerClient
import com.rhadp.parking.data.ParkingAdapterClient
import com.rhadp.parking.data.ServiceException
import com.rhadp.parking.model.SessionInfo
import com.rhadp.parking.ui.session.SessionDashboardViewModel
import com.rhadp.parking.ui.session.SessionDashboardViewModel.UiState
import io.mockk.coEvery
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * Unit tests for [SessionDashboardViewModel].
 *
 * Verifies state transitions for session monitoring:
 * - WaitingForSession → SessionActive (SessionActive = true, polling starts)
 * - SessionActive → SessionCompleted (SessionActive = false)
 * - Connection-lost handling during polling
 *
 * Tests Property 3 (Session State Consistency): UI state matches
 * the most recent SessionActive value from DATA_BROKER.
 *
 * Tests Property 4 (Error Visibility): all failures produce visible errors.
 *
 * Requirements: 06-REQ-3.1–3.4, 06-REQ-3.E1, 06-REQ-7.1, 06-REQ-7.3
 */
@OptIn(ExperimentalCoroutinesApi::class)
class SessionDashboardViewModelTest {

    private val testDispatcher = StandardTestDispatcher()

    private lateinit var dataBrokerClient: DataBrokerClient
    private lateinit var adapterClient: ParkingAdapterClient
    private lateinit var viewModel: SessionDashboardViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        dataBrokerClient = mockk()
        adapterClient = mockk()
        // Use 100ms poll interval for fast tests
        viewModel = SessionDashboardViewModel(dataBrokerClient, adapterClient, 100L)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // --- startMonitoring: normal flow ---

    @Test
    fun `initial state is WaitingForSession`() {
        assertEquals(UiState.WaitingForSession, viewModel.uiState.value)
    }

    @Test
    fun `startMonitoring transitions to SessionActive when signal is true (Property 3)`() =
        runTest {
            val sessionInfo = SessionInfo(
                sessionId = "session-1",
                active = true,
                startTime = 1700000000L,
                currentFee = 0.15,
            )

            every { dataBrokerClient.subscribeSessionActive() } returns flow {
                emit(true)
                // Keep flow alive so polling can happen
                kotlinx.coroutines.awaitCancellation()
            }
            coEvery { adapterClient.getStatus("") } returns sessionInfo

            viewModel.startMonitoring()

            // Advance past initial delay and first poll
            advanceTimeBy(200)

            val state = viewModel.uiState.value
            assertTrue("Expected SessionActive, got $state", state is UiState.SessionActive)
            val activeState = state as UiState.SessionActive
            assertEquals("session-1", activeState.sessionId)
            assertEquals(1700000000L, activeState.startTime)
            assertEquals(0.15, activeState.currentFee, 0.001)
            assertFalse(activeState.connectionLost)
        }

    @Test
    fun `startMonitoring transitions to SessionCompleted when signal goes false`() =
        runTest {
            val finalStatus = SessionInfo(
                sessionId = "session-1",
                active = false,
                startTime = 1700000000L,
                currentFee = 1.50,
            )

            every { dataBrokerClient.subscribeSessionActive() } returns flow {
                emit(false)
            }
            coEvery { adapterClient.getStatus("") } returns finalStatus

            viewModel.startMonitoring()
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue("Expected SessionCompleted, got $state", state is UiState.SessionCompleted)
            val completed = state as UiState.SessionCompleted
            assertEquals(1.50, completed.totalFee, 0.001)
            assertEquals("EUR", completed.currency)
        }

    @Test
    fun `startMonitoring handles session start then stop sequence`() = runTest {
        val activeSession = SessionInfo("s1", true, 1700000000L, 0.10)
        val finalSession = SessionInfo("s1", false, 1700000000L, 0.50)

        var getStatusCallCount = 0
        every { dataBrokerClient.subscribeSessionActive() } returns flow {
            emit(true)
            // Wait for a poll to happen
            kotlinx.coroutines.delay(200)
            emit(false)
        }
        coEvery { adapterClient.getStatus("") } answers {
            getStatusCallCount++
            if (getStatusCallCount <= 2) activeSession else finalSession
        }

        viewModel.startMonitoring()

        // Let the true signal and first poll happen
        advanceTimeBy(300)

        // Now advance to let false signal and final GetStatus happen
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected SessionCompleted, got $state", state is UiState.SessionCompleted)
    }

    // --- startMonitoring: waiting state ---

    @Test
    fun `startMonitoring shows WaitingForSession display text (06-REQ-3 4)`() = runTest {
        every { dataBrokerClient.subscribeSessionActive() } returns flow {
            // Don't emit anything — session hasn't started
            kotlinx.coroutines.awaitCancellation()
        }

        viewModel.startMonitoring()
        advanceUntilIdle()

        assertEquals(UiState.WaitingForSession, viewModel.uiState.value)
    }

    // --- startMonitoring: polling ---

    @Test
    fun `startMonitoring polls GetStatus every interval during active session`() = runTest {
        var pollCount = 0
        every { dataBrokerClient.subscribeSessionActive() } returns flow {
            emit(true)
            kotlinx.coroutines.awaitCancellation()
        }
        coEvery { adapterClient.getStatus("") } answers {
            pollCount++
            SessionInfo("s1", true, 1700000000L, pollCount * 0.05)
        }

        viewModel.startMonitoring()

        // With 100ms interval, after 350ms we expect ~3 polls
        advanceTimeBy(350)

        assertTrue("Expected at least 3 polls, got $pollCount", pollCount >= 3)

        val state = viewModel.uiState.value
        assertTrue("Expected SessionActive, got $state", state is UiState.SessionActive)
    }

    // --- startMonitoring: error cases ---

    @Test
    fun `startMonitoring transitions to Error when subscription fails`() = runTest {
        every { dataBrokerClient.subscribeSessionActive() } returns flow {
            throw ServiceException("Unable to read vehicle location")
        }

        viewModel.startMonitoring()
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertEquals(
            "Unable to read vehicle location",
            (state as UiState.Error).message,
        )
    }

    @Test
    fun `startMonitoring shows connection lost when adapter unreachable (06-REQ-3 E1)`() =
        runTest {
            val lastKnown = SessionInfo("s1", true, 1700000000L, 0.20)

            var callCount = 0
            every { dataBrokerClient.subscribeSessionActive() } returns flow {
                emit(true)
                kotlinx.coroutines.awaitCancellation()
            }
            coEvery { adapterClient.getStatus("") } answers {
                callCount++
                if (callCount == 1) {
                    lastKnown
                } else {
                    throw ServiceException("Unable to get parking session status")
                }
            }

            viewModel.startMonitoring()

            // First poll succeeds
            advanceTimeBy(50)

            val firstState = viewModel.uiState.value
            assertTrue(
                "Expected SessionActive, got $firstState",
                firstState is UiState.SessionActive,
            )
            assertFalse((firstState as UiState.SessionActive).connectionLost)

            // Second poll fails — should show last known with connectionLost
            advanceTimeBy(150)

            val errorState = viewModel.uiState.value
            assertTrue(
                "Expected SessionActive with connectionLost, got $errorState",
                errorState is UiState.SessionActive,
            )
            assertTrue((errorState as UiState.SessionActive).connectionLost)
            assertEquals("s1", errorState.sessionId)
            assertEquals(0.20, errorState.currentFee, 0.001)
        }

    @Test
    fun `startMonitoring transitions to Error when adapter unreachable with no prior data`() =
        runTest {
            every { dataBrokerClient.subscribeSessionActive() } returns flow {
                emit(true)
                kotlinx.coroutines.awaitCancellation()
            }
            coEvery { adapterClient.getStatus("") } throws
                ServiceException("Unable to get parking session status")

            viewModel.startMonitoring()
            advanceTimeBy(50)

            val state = viewModel.uiState.value
            assertTrue("Expected Error, got $state", state is UiState.Error)
        }

    @Test
    fun `startMonitoring uses last known session for summary on final GetStatus failure`() =
        runTest {
            val lastKnown = SessionInfo("s1", true, 1700000000L, 0.30)

            var getStatusCallCount = 0
            every { dataBrokerClient.subscribeSessionActive() } returns flow {
                emit(true)
                kotlinx.coroutines.delay(200)
                emit(false)
            }
            coEvery { adapterClient.getStatus("") } answers {
                getStatusCallCount++
                if (getStatusCallCount <= 2) {
                    lastKnown
                } else {
                    throw ServiceException("Adapter offline")
                }
            }

            viewModel.startMonitoring()

            // Let polling happen then false signal
            advanceTimeBy(300)
            advanceUntilIdle()

            val state = viewModel.uiState.value
            assertTrue("Expected SessionCompleted, got $state", state is UiState.SessionCompleted)
            assertEquals(0.30, (state as UiState.SessionCompleted).totalFee, 0.001)
        }
}

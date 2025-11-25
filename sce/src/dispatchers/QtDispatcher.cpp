#include "dispatchers/QtDispatcher.h"

#include <QCoreApplication>
#include <QEvent>
#include <QEventLoop>
#include <QMetaObject>
#include <QObject>
#include <QThread>
#include <QTimer>
#include <QVariant>
#include <iostream>
#include <map>

namespace SCE::Dispatchers {

// Custom event type ID offset
constexpr int SCE_DISPATCH_EVENT_OFFSET = 778;

/**
 * @brief QObject-derived implementation for Qt event integration
 *
 * This class handles Qt event delivery and timer management.
 * It lives on a dedicated QThread for proper event processing.
 */
class QtDispatcherImpl : public QObject {
    Q_OBJECT

public:
    explicit QtDispatcherImpl(QtDispatcher *owner) : QObject(nullptr), owner_(owner) {}

    ~QtDispatcherImpl() override {
        unregisterAllTimers();
    }

    /**
     * @brief Register custom event type with Qt
     * @return true on success, false if registration failed
     */
    bool registerEventType() {
        if (dispatchEventType_ == QEvent::None) {
            int newType = QEvent::registerEventType(static_cast<int>(QEvent::User) + SCE_DISPATCH_EVENT_OFFSET);
            if (newType > 0) {
                dispatchEventType_ = static_cast<QEvent::Type>(newType);
                return true;
            }
            return false;
        }
        return true;  // Already registered
    }

    /**
     * @brief Get registered event type
     */
    QEvent::Type eventType() const {
        return dispatchEventType_;
    }

    /**
     * @brief Post custom event to wake up Qt event loop
     */
    void postDispatchEvent() {
        if (dispatchEventType_ != QEvent::None) {
            QCoreApplication::postEvent(this, new QEvent(dispatchEventType_));
        }
    }

public slots:

    /**
     * @brief Clean up all timers (must be invoked on worker thread)
     *
     * This slot ensures timers are stopped on the correct thread
     * to avoid "QObject::killTimer: Timers cannot be stopped from another thread" warnings.
     */
    void cleanupAllTimersSlot() {
        std::lock_guard<std::mutex> lock(timersMutex_);
        for (auto &pair : nativeTimers_) {
            pair.second->stop();
            pair.second->deleteLater();
        }
        nativeTimers_.clear();
    }

    /**
     * @brief Start a Qt timer (invoked on worker thread)
     */
    void startTimerSlot(int timerID, unsigned int intervalMs, bool periodic) {
        // Create new timer outside lock (QTimer creation doesn't need protection)
        QTimer *timer = new QTimer(this);
        timer->setProperty("sceTimerId", QVariant(timerID));
        connect(timer, &QTimer::timeout, this, &QtDispatcherImpl::onTimerEvent);
        timer->setSingleShot(!periodic);

        // Single lock scope for map operations
        {
            std::lock_guard<std::mutex> lock(timersMutex_);

            // Defensive cleanup: stop any existing timer with this ID
            auto it = nativeTimers_.find(timerID);
            if (it != nativeTimers_.end()) {
                it->second->stop();
                it->second->deleteLater();
                nativeTimers_.erase(it);
            }

            // Store new timer reference
            nativeTimers_[timerID] = timer;
        }

        // Start timer outside lock (QTimer::start is thread-safe)
        timer->start(static_cast<int>(intervalMs));
    }

    /**
     * @brief Stop a Qt timer (invoked on worker thread)
     */
    void stopTimerSlot(int timerID) {
        std::lock_guard<std::mutex> lock(timersMutex_);
        auto it = nativeTimers_.find(timerID);
        if (it != nativeTimers_.end()) {
            it->second->stop();
            it->second->deleteLater();
            nativeTimers_.erase(it);
        }
    }

protected:
    /**
     * @brief Handle custom Qt events
     */
    void customEvent(QEvent *event) override {
        if (owner_ && event->type() == dispatchEventType_) {
            // Dispatch all pending tasks
            owner_->processPendingTasks();
        }
    }

private slots:

    /**
     * @brief Handle timer timeout
     */
    void onTimerEvent() {
        QTimer *timer = qobject_cast<QTimer *>(sender());
        if (!timer || !owner_) {
            return;
        }

        int timerID = timer->property("sceTimerId").toInt();

        // Get callback from owner's timer metadata
        auto callback = owner_->getCallback(timerID);
        if (callback) {
            try {
                callback();
            } catch (const std::exception &e) {
                std::cerr << "QtDispatcher: Timer callback exception: " << e.what() << std::endl;
            } catch (...) {
                std::cerr << "QtDispatcher: Timer callback unknown exception" << std::endl;
            }
        }

        // Check if one-shot timer - remove from maps
        if (timer->isSingleShot()) {
            // Remove from base class runningTimers_
            {
                std::lock_guard<std::mutex> lock(owner_->timerMutex_);
                owner_->runningTimers_.erase(timerID);
            }
            // Remove from native timers
            {
                std::lock_guard<std::mutex> lock(timersMutex_);
                auto it = nativeTimers_.find(timerID);
                if (it != nativeTimers_.end()) {
                    it->second->deleteLater();
                    nativeTimers_.erase(it);
                }
            }
        }
    }

private:
    void unregisterAllTimers() {
        std::lock_guard<std::mutex> lock(timersMutex_);
        for (auto &pair : nativeTimers_) {
            pair.second->stop();
            pair.second->deleteLater();
        }
        nativeTimers_.clear();
    }

    QtDispatcher *owner_;
    static QEvent::Type dispatchEventType_;
    std::map<int, QTimer *> nativeTimers_;
    std::mutex timersMutex_;
};

// Static member initialization
QEvent::Type QtDispatcherImpl::dispatchEventType_ = QEvent::None;

/**
 * @brief Worker thread with its own event loop
 */
class QtWorkerThread : public QThread {
    Q_OBJECT

public:
    QtWorkerThread() : QThread(nullptr), eventLoop_(nullptr), ready_(false) {}

    void run() override {
        // Create event loop for this thread
        QEventLoop loop;
        eventLoop_ = &loop;

        // Signal ready
        {
            std::lock_guard<std::mutex> lock(readyMutex_);
            ready_ = true;
        }
        readyCv_.notify_all();

        // Run event loop
        loop.exec();

        eventLoop_ = nullptr;
    }

    void stopLoop() {
        if (eventLoop_) {
            QMetaObject::invokeMethod(eventLoop_, "quit", Qt::QueuedConnection);
        }
    }

    void waitUntilReady() {
        std::unique_lock<std::mutex> lock(readyMutex_);
        readyCv_.wait(lock, [this] { return ready_; });
    }

private:
    QEventLoop *eventLoop_;
    std::mutex readyMutex_;
    std::condition_variable readyCv_;
    bool ready_;
};

/**
 * @brief Private implementation data for QtDispatcher
 */
class QtDispatcher::Impl {
public:
    std::unique_ptr<QtDispatcherImpl> qtImpl;
    std::unique_ptr<QtWorkerThread> workerThread;
};

// QtDispatcher implementation

std::shared_ptr<QtDispatcher> QtDispatcher::create() {
    return std::make_shared<QtDispatcher>();
}

QtDispatcher::QtDispatcher() : pImpl_(std::make_unique<Impl>()) {}

QtDispatcher::~QtDispatcher() {
    stop();
    pImpl_.reset();
}

void QtDispatcher::start() {
    if (running_.load()) {
        return;  // Already running
    }

    if (!QCoreApplication::instance()) {
        throw std::runtime_error("QtDispatcher: QCoreApplication must be created before start()");
    }

    // Create and start worker thread first
    pImpl_->workerThread = std::make_unique<QtWorkerThread>();
    pImpl_->workerThread->start();
    pImpl_->workerThread->waitUntilReady();

    // Create Qt implementation object
    pImpl_->qtImpl = std::make_unique<QtDispatcherImpl>(this);

    // Register event type
    if (!pImpl_->qtImpl->registerEventType()) {
        pImpl_->workerThread->stopLoop();
        pImpl_->workerThread->wait();
        pImpl_->workerThread.reset();
        pImpl_->qtImpl.reset();
        throw std::runtime_error("QtDispatcher: Failed to register Qt event type");
    }

    // Move QtDispatcherImpl to worker thread
    pImpl_->qtImpl->moveToThread(pImpl_->workerThread.get());

    running_.store(true);
    stopRequested_.store(false);
}

void QtDispatcher::stop() {
    // Set flags early
    running_.store(false);
    EventDispatcherBase::stop();

    // Clean up timers on worker thread BEFORE stopping it
    // This avoids "QObject::killTimer: Timers cannot be stopped from another thread" warnings
    // (Verified: without this, DestructorStopsTimerThread test shows warnings)
    if (pImpl_->qtImpl && pImpl_->workerThread && pImpl_->workerThread->isRunning()) {
        QMetaObject::invokeMethod(pImpl_->qtImpl.get(), "cleanupAllTimersSlot", Qt::BlockingQueuedConnection);
    }

    // Stop worker thread
    if (pImpl_->workerThread && pImpl_->workerThread->isRunning()) {
        pImpl_->workerThread->stopLoop();
        if (!pImpl_->workerThread->wait(5000)) {
            pImpl_->workerThread->terminate();
            pImpl_->workerThread->wait();
        }
    }

    // Clean up Qt implementation
    pImpl_->qtImpl.reset();
    pImpl_->workerThread.reset();
}

void QtDispatcher::enqueue(std::function<void()> task) {
    if (!running_.load()) {
        throw std::runtime_error("QtDispatcher: Cannot enqueue task, dispatcher not started");
    }

    {
        std::lock_guard<std::mutex> lock(mutex_);
        taskQueue_.push_back(std::move(task));
    }

    notifyDispatcherAboutEvent();
}

void QtDispatcher::run() {
    // Check if dispatcher is valid for running
    if (!running_.load()) {
        if (stopRequested_.load()) {
            return;
        }
        throw std::runtime_error("QtDispatcher: Cannot run, dispatcher not started");
    }

    // Qt event loop runs in worker thread, this just waits for stop
    while (!stopRequested_.load()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    running_.store(false);
}

QtDispatcherImpl *QtDispatcher::getImpl() const {
    return pImpl_ ? pImpl_->qtImpl.get() : nullptr;
}

void QtDispatcher::startTimerImpl(int timerID, unsigned int intervalMs, bool periodic) {
    if (pImpl_ && pImpl_->qtImpl) {
        QMetaObject::invokeMethod(pImpl_->qtImpl.get(), "startTimerSlot", Qt::QueuedConnection, Q_ARG(int, timerID),
                                  Q_ARG(unsigned int, intervalMs), Q_ARG(bool, periodic));
    }
}

void QtDispatcher::stopTimerImpl(int timerID) {
    if (pImpl_ && pImpl_->qtImpl) {
        QMetaObject::invokeMethod(pImpl_->qtImpl.get(), "stopTimerSlot", Qt::QueuedConnection, Q_ARG(int, timerID));
    }
}

void QtDispatcher::notifyDispatcherAboutEvent() {
    if (pImpl_ && pImpl_->qtImpl) {
        pImpl_->qtImpl->postDispatchEvent();
    }
}

}  // namespace SCE::Dispatchers

// Include MOC-generated file at the end
#include "QtDispatcher.moc"

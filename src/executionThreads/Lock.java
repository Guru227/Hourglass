package executionThreads;

public class Lock {
	private boolean isLocked = false;
	private boolean flag = true;
	public synchronized void lock(boolean f) throws InterruptedException {
		while (isLocked || f == flag) {
			wait();
		}
		isLocked = true;
	}	
	public synchronized void unlockAndSet(boolean f) {
		isLocked = false;
		this.flag = f;
		notify();
	}
}

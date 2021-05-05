package executionThreads;

class TimerThread extends Thread{
	Lock lock;
	Timer myTimer;
	//this flag ensures correct starting order of execution and no consecutive execution
	//The latter is ensured by comparing this flag with the flag set in the lock
	boolean flagOfOrder = false;
	
	public TimerThread(Lock l) {
		this.lock = l;
		this.myTimer = new Timer();
	}
	
	public void run() {
		while(true) {
			try {
				lock.lock(flagOfOrder);
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
			myTimer.startTimer();
			System.out.println("Timer has finished running");
			lock.unlockAndSet(flagOfOrder);
		}
	}
}
